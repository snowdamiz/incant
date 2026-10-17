//! Test-only textureLoad copies of actual retained inputs/history, no filtering.
use crate::Renderer;
pub(super) const INPUT: usize = 48;
pub(super) const OUTPUT: usize = 64;
const PLANE: usize = OUTPUT * OUTPUT;
pub(super) const COPY_SHADER: &str = r#"
@group(0) @binding(0) var color:texture_2d<f32>;
@group(0) @binding(1) var motion:texture_2d<f32>;
@group(0) @binding(2) var depth:texture_depth_2d;
@group(0) @binding(3) var history:texture_2d<f32>;
@group(0) @binding(4) var<storage,read_write> output:array<vec4f>;
@compute @workgroup_size(64) fn copy(@builtin(global_invocation_id) id:vec3u) {
    let i=id.x;
    if i<2304u {
        let p=vec2i(i%48u,i/48u);
        output[i]=textureLoad(color,p,0);
        output[4096u+i]=textureLoad(motion,p,0);
        output[8192u+i]=vec4f(textureLoad(depth,p,0),0,0,0);
    }
    if i<4096u { output[12288u+i]=textureLoad(history,vec2i(i%64u,i/64u),0); }
}
"#;
pub(super) struct Pixels(pub Vec<[f32; 4]>);
impl Pixels {
    pub fn current(&self) -> &[[f32; 4]] {
        &self.0[..INPUT * INPUT]
    }
    pub fn motion(&self) -> &[[f32; 4]] {
        &self.0[PLANE..PLANE + INPUT * INPUT]
    }
    pub fn depth(&self) -> &[[f32; 4]] {
        &self.0[2 * PLANE..2 * PLANE + INPUT * INPUT]
    }
    pub fn history(&self) -> &[[f32; 4]] {
        &self.0[3 * PLANE..]
    }
}
pub(super) struct Reader(wgpu::ComputePipeline);
impl Reader {
    pub fn new(r: &Renderer) -> Self {
        let d = &r.device;
        let shader = d.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("TAAU raster raw copy"),
            source: wgpu::ShaderSource::Wgsl(COPY_SHADER.into()),
        });
        let mut entries: Vec<_> = (0..4)
            .map(|binding| wgpu::BindGroupLayoutEntry {
                binding,
                visibility: wgpu::ShaderStages::COMPUTE,
                count: None,
                ty: wgpu::BindingType::Texture {
                    sample_type: if binding == 2 {
                        wgpu::TextureSampleType::Depth
                    } else {
                        wgpu::TextureSampleType::Float { filterable: false }
                    },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
            })
            .collect();
        entries.push(wgpu::BindGroupLayoutEntry {
            binding: 4,
            visibility: wgpu::ShaderStages::COMPUTE,
            count: None,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Storage { read_only: false },
                has_dynamic_offset: false,
                min_binding_size: wgpu::BufferSize::new((4 * PLANE * 16) as u64),
            },
        });
        let layout = d.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &entries,
        });
        let layout = d.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        Self(d.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("TAAU raw copies"),
            layout: Some(&layout),
            module: &shader,
            entry_point: Some("copy"),
            compilation_options: Default::default(),
            cache: None,
        }))
    }
    pub fn read(
        &self,
        r: &Renderer,
        current: &wgpu::TextureView,
        motion: &wgpu::TextureView,
        depth: &wgpu::TextureView,
        history: &wgpu::TextureView,
    ) -> Pixels {
        for v in [current, motion, depth] {
            assert_eq!([v.texture().width(), v.texture().height()], [48, 48]);
        }
        assert_eq!(
            [history.texture().width(), history.texture().height()],
            [64, 64]
        );
        let d = &r.device;
        let bytes = (4 * PLANE * 16) as u64;
        let buffer = d.create_buffer(&wgpu::BufferDescriptor {
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
        let mut entries: Vec<_> = [current, motion, depth, history]
            .iter()
            .enumerate()
            .map(|(i, v)| wgpu::BindGroupEntry {
                binding: i as u32,
                resource: wgpu::BindingResource::TextureView(v),
            })
            .collect();
        entries.push(wgpu::BindGroupEntry {
            binding: 4,
            resource: buffer.as_entire_binding(),
        });
        let group = d.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &self.0.get_bind_group_layout(0),
            entries: &entries,
        });
        let mut encoder = d.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_compute_pass(&Default::default());
            pass.set_pipeline(&self.0);
            pass.set_bind_group(0, &group, &[]);
            pass.dispatch_workgroups(64, 1, 1);
        }
        encoder.copy_buffer_to_buffer(&buffer, 0, &readback, 0, bytes);
        r.queue.submit([encoder.finish()]);
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
        let data = map
            .as_chunks::<16>()
            .0
            .iter()
            .map(|p| {
                std::array::from_fn(|i| f32::from_le_bytes(p[i * 4..i * 4 + 4].try_into().unwrap()))
            })
            .collect();
        drop(map);
        readback.unmap();
        Pixels(data)
    }
}
