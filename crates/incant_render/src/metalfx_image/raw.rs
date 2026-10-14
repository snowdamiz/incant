//! Actual vendor and guard observations, appended to existing display maps.
use super::{HEIGHT, WIDTH, capture};
use crate::{
    Renderer,
    metalfx::diagnostic::{Plane, Snapshot},
};
use serde_json::{Value, json};
use std::time::Duration;
use wgpu::util::DeviceExt;

const MAX_MAP_BYTES: u64 = 25 * 1024 * 1024;
const PLANES: usize = 12; // display + ten retained raw planes + guard mask.
const MASK_BITS: u8 = 63;

pub(super) struct Probe {
    pipeline: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
    mask: wgpu::Texture,
    pub records: Vec<Value>,
}
pub(super) fn requested() -> bool {
    match std::env::var("INCANT_METALFX_RAW_DIAGNOSTIC") {
        Err(std::env::VarError::NotPresent) => false,
        Ok(value) if value == "1" => true,
        _ => panic!("raw diagnostic mode requires exact value1 or absent"),
    }
}
fn shader() -> String {
    let normalize = include_str!("../metalfx/normalize.wgsl");
    assert_eq!(
        normalize
            .matches("if opaque && finite4(n) && all(n.rgb>=vec3f(0.0))")
            .count(),
        1
    );
    assert_eq!(
        normalize
            .matches("full_coverage=full_coverage && admitted(c) && c.a==1.0;")
            .count(),
        1
    );
    assert_eq!(
        normalize
            .matches("opaque=opaque && compatible(linear_depth(textureLoad(depth,tap,0)),z);")
            .count(),
        1
    );
    [
        include_str!("../metalfx/common.wgsl"),
        include_str!("../metalfx/alpha.wgsl"),
        normalize,
        include_str!("guard_mask.wgsl"),
    ]
    .join("\n")
}
#[derive(Debug, PartialEq, Eq)]
struct Layout {
    offset: u64,
    stride: u32,
    size: u64,
}
fn layout(rows: &[(u32, u32, u32)]) -> (Vec<Layout>, u64) {
    assert_eq!(rows.len(), PLANES);
    let mut offset = 0u64;
    let result = rows
        .iter()
        .map(|&(w, h, bytes)| {
            assert!(w > 0 && w <= WIDTH && h > 0 && h <= HEIGHT);
            assert!([1, 2, 4, 8, 16].contains(&bytes));
            let stride = w.checked_mul(bytes).unwrap().div_ceil(256) * 256;
            let size = u64::from(stride) * u64::from(h);
            let row = Layout {
                offset,
                stride,
                size,
            };
            offset = offset.checked_add(size).unwrap();
            assert!(offset <= MAX_MAP_BYTES);
            row
        })
        .collect();
    (result, offset)
}
impl Probe {
    /// Shader/pipeline and mask allocation happen before the scene chronology.
    pub fn new(r: &Renderer, output: &mut capture::Output) -> Self {
        let code = shader();
        output.write("raw/guard-mask.wgsl", code.as_bytes());
        let layout = r
            .device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Read-only MetalFX guard mask"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: wgpu::BufferSize::new(96),
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: false },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: false },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 3,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Depth,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                ],
            });
        let pl = r
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Diagnostic mask only"),
                bind_group_layouts: &[Some(&layout)],
                immediate_size: 0,
            });
        let module = r.device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Unchanged production source plus isolated diagnostic mask"),
            source: wgpu::ShaderSource::Wgsl(code.into()),
        });
        let pipeline = r
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("MetalFX decision observation, never displayed"),
                layout: Some(&pl),
                vertex: wgpu::VertexState {
                    module: &module,
                    entry_point: Some("vertex"),
                    compilation_options: Default::default(),
                    buffers: &[],
                },
                primitive: Default::default(),
                depth_stencil: None,
                multisample: Default::default(),
                fragment: Some(wgpu::FragmentState {
                    module: &module,
                    entry_point: Some("guard_mask"),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: wgpu::TextureFormat::R8Uint,
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                multiview_mask: None,
                cache: None,
            });
        let mask = r.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Six decision bits only"),
            size: wgpu::Extent3d {
                width: WIDTH,
                height: HEIGHT,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R8Uint,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        Self {
            pipeline,
            layout,
            mask,
            records: Vec::with_capacity(4),
        }
    }
    pub fn read(
        &mut self,
        r: &Renderer,
        display: &wgpu::Texture,
        snapshot: &Snapshot,
        arm: &str,
        frame: u32,
        output: &mut capture::Output,
    ) -> Vec<u8> {
        assert!(matches!(arm, "mfx50" | "mfx75") && matches!(frame, 16 | 24));
        assert!(self.records.len() < 4);
        assert_eq!(snapshot.sequence, u64::from(frame));
        assert!(!snapshot.reset);
        assert_eq!(snapshot.output(), [WIDTH, HEIGHT]);
        let settings = snapshot
            .native_settings
            .expect("actual native getters after encode");
        assert_eq!(&settings[..3], &[1., 1., 1.]);
        assert_eq!(
            &settings[3..5],
            &crate::temporal::jitter::SAMPLES[((frame - 1) % 8) as usize]
        );
        assert_eq!(&settings[5..7], &snapshot.content().map(|x| x as f32));
        assert_eq!(&settings[7..], &[0., 0.]);
        let views = snapshot.mask_views();
        let uniform = r
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Exact captured production Frame96"),
                contents: bytemuck::cast_slice(&snapshot.frame),
                usage: wgpu::BufferUsages::UNIFORM,
            });
        let group = r.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Read-only guard inputs"),
            layout: &self.layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&views[0]),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&views[1]),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(&views[2]),
                },
            ],
        });
        let mut planes = Vec::with_capacity(PLANES);
        planes.push(Plane::new("display", display, [0; 2], [WIDTH, HEIGHT]));
        planes.extend(snapshot.planes());
        planes.push(Plane::new(
            "guard-mask",
            &self.mask,
            [0; 2],
            [WIDTH, HEIGHT],
        ));
        let (rows, bytes) = layout(
            &planes
                .iter()
                .map(|p| {
                    let (_, size) = p.copy_rect();
                    (size[0], size[1], p.bytes_per_pixel)
                })
                .collect::<Vec<_>>(),
        );
        let buffer = r.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Bounded12-plane MetalFX observation"),
            size: bytes,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let mut encoder = r.device.create_command_encoder(&Default::default());
        let mask_view = self.mask.create_view(&Default::default());
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Observe guard decisions only"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &mask_view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                // No timestamp queries: map completion is the fence, not a timer.
                ..Default::default()
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &group, &[]);
            pass.draw(0..3, 0..1);
        }
        for (plane, row) in planes.iter().zip(&rows) {
            let (origin, size) = plane.copy_rect();
            encoder.copy_texture_to_buffer(
                wgpu::TexelCopyTextureInfo {
                    texture: &plane.texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d {
                        x: origin[0],
                        y: origin[1],
                        z: 0,
                    },
                    aspect: plane.aspect,
                },
                wgpu::TexelCopyBufferInfo {
                    buffer: &buffer,
                    layout: wgpu::TexelCopyBufferLayout {
                        offset: row.offset,
                        bytes_per_row: Some(row.stride),
                        rows_per_image: Some(size[1]),
                    },
                },
                wgpu::Extent3d {
                    width: size[0],
                    height: size[1],
                    depth_or_array_layers: 1,
                },
            );
        }
        let submitted = r.queue.submit([encoder.finish()]);
        let (tx, rx) = std::sync::mpsc::channel();
        buffer
            .slice(..)
            .map_async(wgpu::MapMode::Read, move |result| {
                let _ = tx.send(result);
            });
        r.device
            .poll(wgpu::PollType::Wait {
                submission_index: Some(submitted),
                timeout: Some(Duration::from_secs(10)),
            })
            .unwrap();
        rx.recv_timeout(Duration::from_secs(10)).unwrap().unwrap();
        let mapped = buffer.slice(..).get_mapped_range();
        assert_eq!(mapped.len() as u64, bytes);
        let mut pixels = Vec::new();
        let mut records = Vec::with_capacity(PLANES);
        let mut mask_counts = [0u64; 64];
        for (plane, row) in planes.iter().zip(&rows) {
            let slice = &mapped[row.offset as usize..(row.offset + row.size) as usize];
            let (copy_origin, copy_size) = plane.copy_rect();
            let crop = [
                plane.origin[0] - copy_origin[0],
                plane.origin[1] - copy_origin[1],
            ];
            let begin = (crop[0] * plane.bytes_per_pixel) as usize;
            let end = begin + (plane.size[0] * plane.bytes_per_pixel) as usize;
            let raw: Vec<_> = slice
                .chunks_exact(row.stride as usize)
                .skip(crop[1] as usize)
                .take(plane.size[1] as usize)
                .flat_map(|r| r[begin..end].iter().copied())
                .collect();
            assert_eq!(
                raw.len(),
                (plane.size[0] * plane.size[1] * plane.bytes_per_pixel) as usize
            );
            if plane.name == "display" {
                pixels = raw;
                continue;
            }
            if plane.name == "guard-mask" {
                for &value in &raw {
                    assert_eq!(value & !MASK_BITS, 0);
                    mask_counts[usize::from(value)] += 1;
                }
            }
            let path = format!("raw/{arm}-{frame:02}/{}.bin", plane.name);
            output.write(&path, &raw);
            records.push(json!({"name":plane.name,"path":path,"format":format!("{:?}",plane.texture.format()),"origin":plane.origin,
                "size":plane.size,"copy_origin":copy_origin,"copy_size":copy_size,"bytes_per_pixel":plane.bytes_per_pixel,"bytes":raw.len(),"sha256":capture::hash(&raw),"little_endian":true}));
        }
        drop(mapped);
        buffer.unmap();
        assert_eq!(records.len(), PLANES - 1);
        assert_eq!(
            mask_counts.iter().sum::<u64>(),
            u64::from(WIDTH) * u64::from(HEIGHT)
        );
        output.write(
            &format!("raw/{arm}-{frame:02}/frame.bin"),
            bytemuck::cast_slice(&snapshot.frame),
        );
        output.write(
            &format!("raw/{arm}-{frame:02}/motion-frame.bin"),
            bytemuck::cast_slice(&snapshot.motion_frame),
        );
        self.records.push(json!({"arm":arm,"frame":frame,"planes":records,"map_bytes":bytes,"map_count":1,"timestamp_query_count":0,
            "mask_counts":mask_counts.as_slice(),"frame_values":snapshot.frame.as_slice(),"motion_frame_values":snapshot.motion_frame.as_slice(),
            "native_getters":settings,"native_getters_order":["preExposure","motionScaleX","motionScaleY","jitterX","jitterY","contentWidth","contentHeight","reset","depthReversed"],
            "completion":"specific queue submission poll and successful full-range map callback, not timing"}));
        pixels
    }
}

#[test]
fn raw_shader_is_separate_and_copy_layout_is_bounded() {
    let code = shader();
    assert!(code.contains(include_str!("../metalfx/normalize.wgsl")));
    let module = naga::front::wgsl::parse_str(&code).unwrap();
    naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::empty(),
    )
    .validate(&module)
    .unwrap();
    for (w, h, expected_bytes) in [(480, 270, 18_386_176), (720, 405, 25_229_056)] {
        let input = [
            (w, h, 8),
            (w, h, 16),
            (WIDTH, HEIGHT, 4),
            (w, h, 8),
            (w, h, 4),
            (w, h, 4),
            (w, h, 1),
        ];
        let mut rows = vec![(WIDTH, HEIGHT, 4)];
        rows.extend(input);
        rows.extend([
            (WIDTH, HEIGHT, 8),
            (WIDTH, HEIGHT, 8),
            (1, 1, 2),
            (WIDTH, HEIGHT, 1),
        ]);
        let (actual, total) = layout(&rows);
        assert_eq!(actual.len(), PLANES);
        assert_eq!(total, expected_bytes);
        assert!(total <= MAX_MAP_BYTES);
        for pair in actual.windows(2) {
            assert_eq!(pair[0].offset + pair[0].size, pair[1].offset);
        }
        assert!(
            actual
                .iter()
                .all(|r| r.offset % 256 == 0 && r.stride % 256 == 0)
        );
    }
}
