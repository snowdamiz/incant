//! Exact sample-index loads into fixed-size inert records; never resolves depth.
use super::{Probe, SIZE};
use crate::Renderer;
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct Sample {
    pub motion: [f32; 4],
    pub position_depth: [f32; 4],
    pub owner: [u32; 4],
}
pub(super) const COPY: &str = r#"
@group(0) @binding(0) var motion_xy:texture_multisampled_2d<u32>;
@group(0) @binding(1) var motion_zw:texture_multisampled_2d<u32>;
@group(0) @binding(2) var owner:texture_multisampled_2d<u32>;
@group(0) @binding(3) var position:texture_multisampled_2d<u32>;
@group(0) @binding(4) var depth:texture_depth_multisampled_2d;
struct Sample {motion:vec4f,position_depth:vec4f,owner:vec4u}
@group(0) @binding(5) var<storage,read_write> output:array<Sample>;
fn unpack(v:vec4u)->vec2f {return bitcast<vec2f>(vec2u(v.x|(v.y<<16u),v.z|(v.w<<16u)));}
@compute @workgroup_size(64) fn copy(@builtin(global_invocation_id) id:vec3u) {
    let i=id.x;if i>=16384u {return;}
    let pixel=i/4u;let s=i32(i%4u);let p=vec2i(vec2u(pixel%64u,pixel/64u));
    let o=textureLoad(owner,p,s).xy;
    output[i]=Sample(vec4f(unpack(textureLoad(motion_xy,p,s)),unpack(textureLoad(motion_zw,p,s))),
        vec4f(unpack(textureLoad(position,p,s)),textureLoad(depth,p,s),f32(s)),vec4u(o.x|(o.y<<16u),0u,0u,0u));
}
"#;
pub(super) struct Reader(wgpu::ComputePipeline, super::Storage);
impl Reader {
    pub fn new(d: &wgpu::Device, arm: Option<&super::diagnostic::Arm>) -> Self {
        let storage = arm.map_or(super::Storage::Original, |a| a.storage);
        let text = arm.map_or_else(
            || COPY.to_string(),
            |arm| {
                if storage == super::Storage::Original {
                    super::diagnostic::copy(arm.explicit)
                } else {
                    super::storage::copy(storage)
                }
            },
        );
        let m = d.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Per-sample visibility readback"),
            source: wgpu::ShaderSource::Wgsl(text.clone().into()),
        });
        let mut entries: Vec<_> = (0..5)
            .map(|binding| wgpu::BindGroupLayoutEntry {
                binding,
                visibility: wgpu::ShaderStages::COMPUTE,
                count: None,
                ty: wgpu::BindingType::Texture {
                    sample_type: if binding == 4 {
                        wgpu::TextureSampleType::Depth
                    } else {
                        wgpu::TextureSampleType::Uint
                    },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: true,
                },
            })
            .collect();
        entries.push(wgpu::BindGroupLayoutEntry {
            binding: 5,
            visibility: wgpu::ShaderStages::COMPUTE,
            count: None,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Storage { read_only: false },
                has_dynamic_offset: false,
                min_binding_size: wgpu::BufferSize::new(Self::bytes(storage)),
            },
        });
        let group = d.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &entries,
        });
        let layout = d.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[Some(&group)],
            immediate_size: 0,
        });
        let (pipeline, msl) = super::msl::capture(arm.is_some(), &[("copy", "Compute")], || {
            d.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: None,
                layout: Some(&layout),
                module: &m,
                entry_point: Some("copy"),
                compilation_options: Default::default(),
                cache: None,
            })
        });
        if let Some(arm) = arm {
            super::diagnostic::write(
                &arm.root,
                "pipeline-readback.json",
                &serde_json::json!({"WGSL":text,"MSL":msl}),
            );
            assert_eq!(msl["complete"], true, "actual readback MSL required");
        }
        Self(pipeline, storage)
    }
    fn bytes(storage: super::Storage) -> u64 {
        u64::from(SIZE * SIZE * 4)
            * if storage == super::Storage::Original {
                48
            } else {
                80
            }
    }
    fn read_bytes(&self, r: &Renderer, probe: &Probe) -> Vec<u8> {
        let d = &r.device;
        let bytes = Self::bytes(self.1);
        let storage = d.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: bytes,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let readback = d.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: bytes,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let slots = self.1.readback_slots();
        let colors = slots.map(|slot| &probe.colors[slot]);
        let mut entries: Vec<_> = colors
            .into_iter()
            .chain([&probe.depth])
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
        let group = d.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &self.0.get_bind_group_layout(0),
            entries: &entries,
        });
        let mut e = d.create_command_encoder(&Default::default());
        {
            let mut p = e.begin_compute_pass(&Default::default());
            p.set_pipeline(&self.0);
            p.set_bind_group(0, &group, &[]);
            p.dispatch_workgroups(256, 1, 1);
        }
        e.copy_buffer_to_buffer(&storage, 0, &readback, 0, bytes);
        r.queue.submit([e.finish()]);
        let (tx, rx) = std::sync::mpsc::channel();
        readback.slice(..).map_async(wgpu::MapMode::Read, move |v| {
            let _ = tx.send(v);
        });
        d.poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: Some(std::time::Duration::from_secs(10)),
        })
        .unwrap();
        rx.recv_timeout(std::time::Duration::from_secs(10))
            .unwrap()
            .unwrap();
        let map = readback.slice(..).get_mapped_range();
        let result = map.to_vec();
        drop(map);
        readback.unmap();
        result
    }
    pub fn read(&self, r: &Renderer, probe: &Probe) -> Vec<Sample> {
        assert_eq!(self.1, super::Storage::Original);
        self.read_bytes(r, probe)
            .as_chunks::<48>()
            .0
            .iter()
            .map(|b| parse(b))
            .collect()
    }
    pub fn read_storage(&self, r: &Renderer, probe: &Probe) -> Vec<super::StorageRecord> {
        assert_ne!(self.1, super::Storage::Original);
        self.read_bytes(r, probe)
            .as_chunks::<80>()
            .0
            .iter()
            .map(|b| super::StorageRecord {
                sample: parse(&b[..48]),
                raw_motion_zw: std::array::from_fn(|i| {
                    u32::from_le_bytes(b[48 + i * 4..52 + i * 4].try_into().unwrap())
                }),
                raw_owner: std::array::from_fn(|i| {
                    u32::from_le_bytes(b[64 + i * 4..68 + i * 4].try_into().unwrap())
                }),
            })
            .collect()
    }
}

fn parse(b: &[u8]) -> Sample {
    Sample {
        motion: std::array::from_fn(|i| {
            f32::from_le_bytes(b[i * 4..4 + i * 4].try_into().unwrap())
        }),
        position_depth: std::array::from_fn(|i| {
            f32::from_le_bytes(b[16 + i * 4..20 + i * 4].try_into().unwrap())
        }),
        owner: std::array::from_fn(|i| {
            u32::from_le_bytes(b[32 + i * 4..36 + i * 4].try_into().unwrap())
        }),
    }
}
