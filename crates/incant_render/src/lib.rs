//! Phase 0 native-surface and actual-GPU screenshot proof, not a production PBR renderer.
use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Quat, Vec3};
use incant_doc::{Project, Transform};
use std::{collections::HashMap, error::Error, sync::Mutex, time::Duration};
pub use wgpu;
use wgpu::util::DeviceExt;
type Result<T> = std::result::Result<T, Box<dyn Error + Send + Sync>>;
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Vertex {
    position: [f32; 4],
    normal: [f32; 3],
}
/// Physical-pixel viewport placement supplied by the editor, including its chrome mask.
#[derive(Clone, Copy)]
pub struct Viewport {
    pub rect: [f32; 4],
    pub corner_radii: [f32; 4],
    pub canvas_srgb: [u8; 3],
}
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct ClipUniform {
    rect: [f32; 4],
    radii: [f32; 4],
    canvas: [f32; 4],
}
pub struct Renderer {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub adapter: wgpu::Adapter,
    pub adapter_name: String,
    pipelines: Mutex<HashMap<(wgpu::TextureFormat, bool), wgpu::RenderPipeline>>,
}
impl Renderer {
    pub async fn new(
        instance: &wgpu::Instance,
        surface: Option<&wgpu::Surface<'_>>,
    ) -> Result<Self> {
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::LowPower,
                compatible_surface: surface,
                force_fallback_adapter: false,
            })
            .await?;
        let adapter_name = adapter.get_info().name;
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("Incant Phase 0"),
                ..Default::default()
            })
            .await?;
        Ok(Self {
            device,
            queue,
            adapter,
            adapter_name,
            pipelines: Mutex::new(HashMap::new()),
        })
    }
    pub fn headless() -> Result<Self> {
        let instance = wgpu::Instance::default();
        pollster::block_on(Self::new(&instance, None))
    }
    fn pipeline(&self, format: wgpu::TextureFormat) -> wgpu::RenderPipeline {
        let shader = self
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Phase 0 diagnostic shader"),
                source: wgpu::ShaderSource::Wgsl(include_str!("fixture.wgsl").into()),
            });
        self.device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("Phase 0 geometry"),
                layout: None,
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vertex"),
                    compilation_options: Default::default(),
                    buffers: &[wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<Vertex>() as u64,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &wgpu::vertex_attr_array![0=>Float32x4,1=>Float32x3],
                    }],
                },
                primitive: wgpu::PrimitiveState {
                    cull_mode: None,
                    ..Default::default()
                },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: wgpu::TextureFormat::Depth32Float,
                    depth_write_enabled: Some(true),
                    depth_compare: Some(wgpu::CompareFunction::Less),
                    stencil: Default::default(),
                    bias: Default::default(),
                }),
                multisample: Default::default(),
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some("fragment"),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format,
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                multiview_mask: None,
                cache: None,
            })
    }
    fn clip_pipeline(&self, format: wgpu::TextureFormat) -> wgpu::RenderPipeline {
        let shader = self
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Native viewport corner mask"),
                source: wgpu::ShaderSource::Wgsl(include_str!("viewport_clip.wgsl").into()),
            });
        self.device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("Native viewport composition"),
                layout: None,
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vertex"),
                    compilation_options: Default::default(),
                    buffers: &[],
                },
                primitive: Default::default(),
                depth_stencil: None,
                multisample: Default::default(),
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some("fragment"),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format,
                        blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                multiview_mask: None,
                cache: None,
            })
    }
    fn cached_pipeline(
        &self,
        format: wgpu::TextureFormat,
        clip: bool,
    ) -> Result<wgpu::RenderPipeline> {
        let mut pipelines = self
            .pipelines
            .lock()
            .map_err(|_| "render cache lock failed")?;
        Ok(pipelines
            .entry((format, clip))
            .or_insert_with(|| {
                if clip {
                    self.clip_pipeline(format)
                } else {
                    self.pipeline(format)
                }
            })
            .clone())
    }
    pub fn draw(
        &self,
        project: &Project,
        view: &wgpu::TextureView,
        format: wgpu::TextureFormat,
        width: u32,
        height: u32,
        viewport: Option<Viewport>,
    ) -> Result<wgpu::CommandBuffer> {
        if width == 0 || height == 0 || width > 8192 || height > 8192 {
            return Err("invalid render target size".into());
        }
        project.validate()?;
        let rect = viewport
            .map(|v| v.rect)
            .unwrap_or([0., 0., width as f32, height as f32]);
        if rect.iter().any(|v| !v.is_finite())
            || rect[0] < 0.
            || rect[1] < 0.
            || rect[2] <= 0.
            || rect[3] <= 0.
            || rect[0] + rect[2] > width as f32
            || rect[1] + rect[3] > height as f32
        {
            return Err("viewport outside render target".into());
        }
        if viewport.is_some_and(|v| v.corner_radii.iter().any(|r| !r.is_finite() || *r < 0.)) {
            return Err("invalid viewport corner radius".into());
        }
        let vertices = vertices(project, rect[2] / rect[3])?;
        let buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("ECS diagnostic geometry"),
                contents: if vertices.is_empty() {
                    &[0u8; 28]
                } else {
                    bytemuck::cast_slice(&vertices)
                },
                usage: wgpu::BufferUsages::VERTEX,
            });
        let depth = self
            .device
            .create_texture(&wgpu::TextureDescriptor {
                label: Some("Depth"),
                size: wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Depth32Float,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            })
            .create_view(&Default::default());
        let pipeline = self.cached_pipeline(format, false)?;
        let mut encoder = self.device.create_command_encoder(&Default::default());
        // Claude's neutral viewport backdrop, sRGB #141519. Match the
        // attachment's transfer function just as the composition canvas does.
        let backdrop = [20, 21, 25].map(|channel| {
            let value = f64::from(channel) / 255.;
            if format.is_srgb() {
                ((value + 0.055) / 1.055).powf(2.4)
            } else {
                value
            }
        });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Native viewport"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: backdrop[0],
                            g: backdrop[1],
                            b: backdrop[2],
                            a: 1.,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &depth,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.),
                        store: wgpu::StoreOp::Discard,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&pipeline);
            pass.set_viewport(rect[0], rect[1], rect[2], rect[3], 0., 1.);
            pass.set_vertex_buffer(0, buffer.slice(..));
            pass.draw(0..vertices.len() as u32, 0..1);
        }
        if let Some(viewport) = viewport {
            let linear = viewport.canvas_srgb.map(|v| {
                let v = f32::from(v) / 255.;
                if !format.is_srgb() {
                    v
                } else if v <= 0.04045 {
                    v / 12.92
                } else {
                    ((v + 0.055) / 1.055).powf(2.4)
                }
            });
            let uniform = ClipUniform {
                rect,
                radii: viewport.corner_radii,
                canvas: [linear[0], linear[1], linear[2], 1.],
            };
            let buffer = self
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Viewport clipping bounds"),
                    contents: bytemuck::bytes_of(&uniform),
                    usage: wgpu::BufferUsages::UNIFORM,
                });
            let pipeline = self.cached_pipeline(format, true)?;
            let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Viewport clipping bounds"),
                layout: &pipeline.get_bind_group_layout(0),
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: buffer.as_entire_binding(),
                }],
            });
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Composite the viewport into editor chrome"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                ..Default::default()
            });
            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.draw(0..3, 0..1);
        }
        Ok(encoder.finish())
    }
    pub fn screenshot_png(&self, project: &Project, width: u32, height: u32) -> Result<Vec<u8>> {
        if !(16..=1920).contains(&width) || !(16..=1080).contains(&height) {
            return Err("screenshot size out of bounds".into());
        }
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Screenshot"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        self.queue.submit([self.draw(
            project,
            &texture.create_view(&Default::default()),
            wgpu::TextureFormat::Rgba8UnormSrgb,
            width,
            height,
            None,
        )?]);
        let stride = (width * 4).div_ceil(256) * 256;
        let buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Screenshot readback"),
            size: u64::from(stride) * u64::from(height),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = self.device.create_command_encoder(&Default::default());
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(stride),
                    rows_per_image: Some(height),
                },
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );
        self.queue.submit([encoder.finish()]);
        let (tx, rx) = std::sync::mpsc::channel();
        buffer
            .slice(..)
            .map_async(wgpu::MapMode::Read, move |result| {
                let _ = tx.send(result);
            });
        self.device.poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: Some(Duration::from_secs(10)),
        })?;
        rx.recv_timeout(Duration::from_secs(10))??;
        let data = buffer.slice(..).get_mapped_range();
        let mut pixels = Vec::with_capacity((width * height * 4) as usize);
        for row in data.chunks(stride as usize) {
            pixels.extend_from_slice(&row[..(width * 4) as usize]);
        }
        drop(data);
        buffer.unmap();
        let mut bytes = vec![];
        {
            let mut encoder = png::Encoder::new(&mut bytes, width, height);
            encoder.set_color(png::ColorType::Rgba);
            encoder.set_depth(png::BitDepth::Eight);
            encoder.write_header()?.write_image_data(&pixels)?;
        }
        Ok(bytes)
    }
}
fn vertices(project: &Project, aspect: f32) -> Result<Vec<Vertex>> {
    // The spike consumes a real Bevy ECS query projection. Keep this isolated
    // from authored state; production rendering will cache and update it incrementally.
    let state = incant_core::Engine::new(project)?.snapshot();
    let view = Mat4::look_at_rh(Vec3::new(6., 5., 9.), Vec3::ZERO, Vec3::Y);
    let projection = Mat4::perspective_rh(50f32.to_radians(), aspect, 0.1, 1000.);
    let corners = [
        [-0.5, -0.5, -0.5],
        [0.5, -0.5, -0.5],
        [0.5, 0.5, -0.5],
        [-0.5, 0.5, -0.5],
        [-0.5, -0.5, 0.5],
        [0.5, -0.5, 0.5],
        [0.5, 0.5, 0.5],
        [-0.5, 0.5, 0.5],
    ];
    let faces = [
        ([0, 1, 2, 0, 2, 3], [0., 0., -1.]),
        ([4, 6, 5, 4, 7, 6], [0., 0., 1.]),
        ([0, 4, 5, 0, 5, 1], [0., -1., 0.]),
        ([3, 2, 6, 3, 6, 7], [0., 1., 0.]),
        ([0, 3, 7, 0, 7, 4], [-1., 0., 0.]),
        ([1, 5, 6, 1, 6, 2], [1., 0., 0.]),
    ];
    let mut output = vec![];
    for scene in project.scenes.values() {
        for entity in scene.entities.values() {
            let Some(value) = entity.components.get("Transform") else {
                continue;
            };
            let t: Transform = serde_json::from_value(value.clone())?;
            let world = Mat4::from_scale_rotation_translation(
                Vec3::from_array(t.scale.map(|x| x as f32)),
                Quat::from_array(t.rotation.map(|x| x as f32)),
                Vec3::from_array(state.entities[&entity.id].translation.map(|x| x as f32)),
            );
            let mvp = projection * view * world;
            for (indices, normal) in faces {
                let normal = world
                    .inverse()
                    .transpose()
                    .transform_vector3(Vec3::from_array(normal))
                    .normalize()
                    .to_array();
                for index in indices {
                    let position = mvp * Vec3::from_array(corners[index]).extend(1.);
                    output.push(Vertex {
                        position: position.to_array(),
                        normal,
                    });
                }
            }
        }
    }
    Ok(output)
}
