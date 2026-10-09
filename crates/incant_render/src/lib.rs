//! Native rendering and GPU readback with retained imported material previews.
//! Ordered HDR geometry, display transform and native composition passes.
//! Clustered lighting and the full production render graph remain open.
mod frame;
#[cfg(test)]
#[path = "../tests/hdr_output/mod.rs"]
mod hdr_output;
mod material_pipeline;
mod materials;
mod models;
mod output;
mod resource_error;
mod scene;
mod studio;
#[cfg(test)]
#[path = "../tests/support/mod.rs"]
mod test_support;
use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec3};
use incant_doc::Project;
pub use models::RenderScene;
pub use resource_error::ResourceError;
pub use scene::{SceneError, SceneStats};
use std::{collections::HashMap, error::Error, sync::Mutex, time::Duration};
pub use wgpu;
use wgpu::util::DeviceExt;
type Result<T> = std::result::Result<T, Box<dyn Error + Send + Sync>>;
pub const MIN_SCREENSHOT_DIMENSION: u32 = 16;
pub const MAX_SCREENSHOT_WIDTH: u32 = 1920;
pub const MAX_SCREENSHOT_HEIGHT: u32 = 1080;
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
pub struct Renderer {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub adapter: wgpu::Adapter,
    pub adapter_name: String,
    pipelines: Mutex<HashMap<wgpu::TextureFormat, wgpu::RenderPipeline>>,
    models: Mutex<models::ModelCache>,
    materials: materials::MaterialSystem,
    frames: frame::FrameCache,
    output: output::OutputPass,
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
                label: Some("Incant renderer"),
                ..Default::default()
            })
            .await?;
        let materials = materials::MaterialSystem::new(&device, &queue);
        Ok(Self {
            device,
            queue,
            adapter,
            adapter_name,
            pipelines: Mutex::new(HashMap::new()),
            models: Mutex::new(HashMap::new()),
            materials,
            frames: Default::default(),
            output: Default::default(),
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
    fn cached_pipeline(&self, format: wgpu::TextureFormat) -> Result<wgpu::RenderPipeline> {
        let mut pipelines = self
            .pipelines
            .lock()
            .map_err(|_| ResourceError::CacheLock("diagnostic pipeline"))?;
        Ok(pipelines
            .entry(format)
            .or_insert_with(|| self.pipeline(format))
            .clone())
    }
    /// Phase 0 compatibility path. Bound models require prepare_scene and draw_scene.
    pub fn draw(
        &self,
        project: &Project,
        view: &wgpu::TextureView,
        format: wgpu::TextureFormat,
        width: u32,
        height: u32,
        viewport: Option<Viewport>,
    ) -> Result<wgpu::CommandBuffer> {
        self.draw_scene(
            &self.diagnostic_scene(project)?,
            view,
            format,
            width,
            height,
            viewport,
        )
    }
    pub fn draw_scene(
        &self,
        scene: &RenderScene,
        view: &wgpu::TextureView,
        format: wgpu::TextureFormat,
        width: u32,
        height: u32,
        viewport: Option<Viewport>,
    ) -> Result<wgpu::CommandBuffer> {
        output::OutputPass::validate(format)?;
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
        let attachments = self.frames.get(&self.device, width, height)?;
        let vertices = vertices(&scene.diagnostics, rect[2] / rect[3]);
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
        let hdr = &attachments.color;
        let depth = &attachments.depth;
        let pipeline = self.cached_pipeline(frame::HDR_FORMAT)?;
        let mut encoder = self.device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Linear HDR geometry"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: hdr,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: depth,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.),
                        store: wgpu::StoreOp::Store,
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
        self.render_models(
            scene,
            models::ModelTarget {
                color: hdr,
                depth,
                format: frame::HDR_FORMAT,
                rect,
            },
            &mut encoder,
        )?;
        self.output
            .encode(&self.device, &mut encoder, hdr, view, format, viewport)?;
        Ok(encoder.finish())
    }
    pub fn screenshot_png(&self, project: &Project, width: u32, height: u32) -> Result<Vec<u8>> {
        self.screenshot_scene_png(&self.diagnostic_scene(project)?, width, height)
    }
    pub fn screenshot_scene_png(
        &self,
        scene: &RenderScene,
        width: u32,
        height: u32,
    ) -> Result<Vec<u8>> {
        if !(MIN_SCREENSHOT_DIMENSION..=MAX_SCREENSHOT_WIDTH).contains(&width)
            || !(MIN_SCREENSHOT_DIMENSION..=MAX_SCREENSHOT_HEIGHT).contains(&height)
        {
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
        self.queue.submit([self.draw_scene(
            scene,
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
fn camera(aspect: f32) -> Mat4 {
    Mat4::perspective_rh(50f32.to_radians(), aspect, 0.1, 1000.)
        * Mat4::look_at_rh(Vec3::from_array(studio::EYE), Vec3::ZERO, Vec3::Y)
}
fn vertices(transforms: &[Mat4], aspect: f32) -> Vec<Vertex> {
    let camera = camera(aspect);
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
    for world in transforms {
        let mvp = camera * world;
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
    output
}
